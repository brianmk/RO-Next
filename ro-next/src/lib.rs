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

// Helper macro to time and print the startup time of RO-Next
mod msgstringtable;

macro_rules! time_phase {
    ($message:expr, { $($statements:tt)* }) => {
        #[cfg(feature = "debug")]
        let _statement_timer = ro_next_debug::logging::Timer::new($message);

        $($statements)*

        #[cfg(feature = "debug")]
        _statement_timer.stop();
    }
}

mod benchmark;
mod client;

use self::client::items::*;
use self::client::render::MapRenderContext;
mod client_config;
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
mod pincode;
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

use cgmath::{InnerSpace, Point3, Vector3};
use hashbrown::HashMap;
use image::{EncodableLayout, ImageFormat, ImageReader};
use input::{MouseInputMode, MouseModeExt};
#[cfg(feature = "debug")]
use networking::{PacketHistory, PacketHistoryCallback};
use ragnarok_packets::{
    AccountId, AmmunitionActionType, AttackRange, BuyItemResult, CharacterId, CharacterServerInformation, ClientTick, Direction,
    DisappearanceReason, EntityId, EquipPosition, EquippableItemFlags, ExperienceSource, ExperienceType, Friend, HotbarSlot, HotkeyType,
    InventoryIndex, ItemId, JobId, MarkerType, SellItemsResult, SkillId, SkillLevel, SkillType, StatType, TilePosition, UnitId,
    WorldPosition,
};
use renderer::InterfaceRenderer;
use ro_next_audio::{AudioEngine, SoundEffectKey};
#[cfg(feature = "debug")]
use ro_next_debug::logging::{Colorize, print_debug};
#[cfg(feature = "debug")]
use ro_next_debug::profile_block;
#[cfg(feature = "debug")]
use ro_next_debug::profiling::Profiler;
use ro_next_interface::application::Clip;
use ro_next_interface::layout::MouseButton;
use ro_next_interface::{Interface, MouseMode, take_layout_profile};
use ro_next_loaders::FileLoader;
use ro_next_messages::{MessageDomain, format_message};
use ro_next_networking::{
    DisconnectReason, HotkeyState, ItemQuantity, LoginServerLoginData, MessageColor, NetworkEvent, NetworkEventBuffer, NetworkingSystem,
    SellItem, SupportedPacketVersion,
};
use rust_state::{ManuallyAssertExt, State};
#[cfg(feature = "debug")]
use rust_state::{VecIndexExt, VecLookupExt};
use settings::{
    AudioSettings, AudioSettingsPathExt, AuraStyle, GraphicsSettingsCapabilities, GraphicsSettingsPathExt, InterfaceSettings,
    InterfaceSettingsPathExt,
};
use state::bank::parse_zeny_amount;
use state::content_windows::ContentWindowsPathExt;
use state::hotbar::{HOTBAR_ENTRIES_PER_TAB, HOTBAR_TAB_COUNT, HotbarPathExt};
use state::inventory::InventoryPathExt;
use state::lifecycle::{
    AddEntityDecision, EntityLifecycle, RemoveEntityOutcome, ResourcePolicy, ResourceRetryQueue, SHUTDOWN_DISCONNECT_TIMEOUT,
    SessionAction, SessionEvent, SessionMachine, SessionObservation, SessionState,
};
use state::localization::Localization;
use state::skills::SkillTreePathExt;
use state::theme::{CursorThemePathExt, IndicatorThemePathExt, InterfaceThemePathExt, WorldThemePathExt};
use state::vending::VendingShopItem;
use state::{ChatChannel, ChatMessage, ClientState, ClientStatePathExt, client_state, status_effect_definition, this_entity, this_player};
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
use winit::keyboard::PhysicalKey;
use winit::window::{Fullscreen, Icon, Window, WindowId};

use crate::graphics::*;
use crate::input::{BattleModeCommand, InputEvent, InputReport, InputSystem, parse_battle_mode_command, translate_slash_command};
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
use crate::state::character_slots::{MAX_CHARACTER_SLOTS, visible_character_slots};
use crate::state::chat_markup::ChatMarkupLink;
use crate::state::inventory::{ItemCategory, ItemUseAction, item_category, item_type, item_use_action};
use crate::state::item_service::{ItemServiceEntry, ItemServiceKind, ItemServicePathExt, RefiningMaterial, RefiningUiPathExt};
use crate::state::localization::{LocalizationPathExt, QolTextsPathExt};
use crate::state::minimap::{MapWindowCycle, direction_index_from_tile_delta};
use crate::state::notifications::NotificationKind;
use crate::state::party::PartyPathExt;
use crate::state::skills::{LearnedSkill, SkillTreeLayoutPathExt, bring_skill_to_level};
use crate::state::stat_buffer::PendingStatUpdates;
use crate::state::theme::{ColorTokensPathExt, DesignTokensPathExt, InterfaceTheme, InterfaceThemeType, WorldTheme};
use crate::state::{BufferedAction, SelectedServicePath, client_theme};
use crate::system::{FrameTimers, GameTimer};
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;
use crate::world::target_policy::{TargetRelations, player_attack_decision, player_combat_zone};
use crate::world::*;

/// Next unique point-light id for position based effects. The point light
/// manager uses the id to decide which light renders with a shadow, so it has
/// to be unique.
static NEXT_POSITION_EFFECT_LIGHT_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0x5000_0000);

/// Packet callback used in release builds.
///
/// Unknown/unparseable 2019 server packets are logged at debug verbosity
/// (`-vv` / `RO_NEXT_LOG_LEVEL=2`) so that missing handlers can be spotted
/// without the debug packet inspector.
#[cfg(not(feature = "debug"))]
#[derive(Clone, Copy, Default)]
struct ClientPacketCallback;

#[cfg(not(feature = "debug"))]
impl ragnarok_packets::handler::PacketCallback for ClientPacketCallback {
    fn incoming_packet<Packet>(&self, _packet: &Packet)
    where
        Packet: ragnarok_packets::Packet,
    {
    }

    fn outgoing_packet<Packet>(&self, _packet: &Packet)
    where
        Packet: ragnarok_packets::Packet,
    {
    }

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
/// Minimum duration of the skill-cast animation for casts without a usable cast
/// time, so that they are still visible.
/// Duration of the one-shot attack/pickup animation for instant skills, where
/// the server sends no attack motion (the official client uses a 400 ms hold).
const INSTANT_SKILL_ANIMATION_MS: u32 = 400;
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
        // Companions use the classic light green/cyan-ish pet names.
        EntityType::Pet | EntityType::Homunculus | EntityType::Mercenary => Color::rgb_u8(0x9A, 0xE8, 0x9A),
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

        match (enabled("RO_NEXT_X11"), enabled("RO_NEXT_WAYLAND")) {
            (true, true) => {
                crate::logging::log_warn!("[display] both RO_NEXT_X11 and RO_NEXT_WAYLAND are set, using X11");
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
/// `EventLoop::new().unwrap()`. Force a backend with `RO_NEXT_X11=1` or
/// `RO_NEXT_WAYLAND=1`.
#[cfg(target_os = "linux")]
fn create_event_loop() -> Result<EventLoop<()>, String> {
    let preference = BackendPreference::from_environment();

    match preference {
        BackendPreference::X11 => {
            let event_loop = create_x11_event_loop().map_err(|error| format!("failed to create the X11 event loop: {error}"))?;
            eprintln!("[display] using X11 backend (forced by RO_NEXT_X11)");
            Ok(event_loop)
        }
        BackendPreference::Wayland => {
            let event_loop = create_wayland_event_loop().map_err(|error| format!("failed to create the Wayland event loop: {error}"))?;
            eprintln!("[display] using native Wayland backend (forced by RO_NEXT_WAYLAND)");
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
ro_next_debug::create_profiler_threads!(threads, {
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

    /// Clears and returns the four aggregated amounts (base, job, quest base,
    /// quest job), or [`None`] when nothing was gained.
    ///
    /// The chat text and its labels are localized by the caller
    /// (`flush_experience_feedback`), so this only reports the amounts.
    fn take_gains(&mut self) -> Option<[i64; 4]> {
        if self.is_empty() {
            self.last_packet = None;
            return None;
        }

        let gains = [self.base, self.job, self.quest_base, self.quest_job];

        self.base = 0;
        self.job = 0;
        self.quest_base = 0;
        self.quest_job = 0;
        self.last_packet = None;

        Some(gains)
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

/// Wire value of [`BuyItemResult`] (`ZC_PC_PURCHASE_RESULT` result byte).
///
/// Only the protocol result code is mapped here; the user-facing texts live in
/// `data\client\messages.ron` (`MessageDomain::BuyItem`).
fn buy_item_result_code(result: &BuyItemResult) -> u16 {
    match result {
        // Handled by the outer `BuyingCompleted` match arm: a successful
        // purchase only closes the windows and never shows a message.
        BuyItemResult::Successful => unreachable!(),
        BuyItemResult::NotEoughZeny => 1,
        BuyItemResult::WeightLimitExceeded => 2,
        BuyItemResult::TooManyItems => 3,
        BuyItemResult::TooManyOfThisItem => 9,
        BuyItemResult::PropsOpenAir => 10,
        BuyItemResult::ExchangeFailed => 11,
        BuyItemResult::ExchangeWellDone => 12,
        BuyItemResult::ItemSoldOut => 13,
        BuyItemResult::NotEnoughGoods => 14,
    }
}

/// Wire value of [`AmmunitionActionType`] (`ZC_ACTION_FAILURE` result, u16).
///
/// Only the protocol result code is mapped here; the user-facing texts live in
/// `data\client\messages.ron` (`MessageDomain::AmmunitionAction`).
fn ammunition_action_code(action: &AmmunitionActionType) -> u16 {
    match action {
        AmmunitionActionType::EquipProperAmmunitionFirst => 0,
        AmmunitionActionType::WeightLimitExceeded1 => 1,
        AmmunitionActionType::WeightLimitExceeded2 => 2,
        AmmunitionActionType::Equipped => 3,
    }
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

/// Loads the archive override of the data-driven protocol result-code message
/// table.
///
/// The client always has the compiled fallback (`ro-next messages` crate,
/// `include_str!` of `data\client\messages.ron`), so a missing or malformed
/// archive override only logs a warning and never leaves result messages
/// unavailable.
fn load_protocol_messages(game_file_loader: &GameFileLoader) {
    use ro_next_loaders::FileLoader;

    let bytes = match game_file_loader.get(ro_next_messages::DATA_PATH) {
        Ok(bytes) => bytes,
        Err(_) => {
            crate::logging::log_debug!(
                "messages: {} not found in the archives, using the compiled fallback",
                ro_next_messages::DATA_PATH
            );
            return;
        }
    };

    let content = String::from_utf8_lossy(&bytes);

    match ro_next_messages::install(&content) {
        Ok(count) => crate::logging::log_debug!("messages: loaded {count} result messages from the archive"),
        Err(error) => crate::logging::log_warn!(
            "messages: invalid {} ({error}), using the compiled fallback",
            ro_next_messages::DATA_PATH
        ),
    }
}

/// Validates the unified skill effect descriptor data and returns the number of
/// `(skill, phase)` descriptors.
///
/// `path` selects a concrete data file (for example an archive override);
/// without a path the compiled fallback
/// (`ro-next/archive/data/client/skill_effects.ron`) is validated. This is the
/// `--check`-able schema entry point: the client binary exposes it as `ro-next
/// --check-skill-effects [PATH]`, so CI can validate the committed data file
/// without starting a window, a network connection or the game archives.
pub fn check_skill_effects(path: Option<&std::path::Path>) -> Result<usize, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            crate::world::validate_skill_effect_data(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => crate::world::validate_compiled_skill_effects().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates the item stats data (`archive/data/client/item_stats.ron`) and
/// returns the number of item rows.
pub fn check_item_stats(path: Option<&std::path::Path>) -> Result<usize, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            crate::world::validate_item_stats_data(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => crate::world::validate_compiled_item_stats().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates the data-driven protocol result-code message table and returns the
/// number of message rows.
pub fn check_messages(path: Option<&std::path::Path>) -> Result<usize, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            ro_next_messages::validate(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => ro_next_messages::validate_compiled().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates the data-driven status-effect table and returns the number of
/// status entries.
pub fn check_status_effects(path: Option<&std::path::Path>) -> Result<usize, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            crate::world::validate_status_effect_data(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => crate::world::validate_compiled_status_effects().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates the data-driven status body-visual table and returns the number of
/// rows.
pub fn check_status_visuals(path: Option<&std::path::Path>) -> Result<usize, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            crate::world::validate_status_visual_data(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => crate::world::validate_compiled_status_visuals().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates the data-driven job/appearance registry and returns the number of
/// job rows.
pub fn check_jobs(path: Option<&std::path::Path>) -> Result<usize, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            crate::world::jobs::validate(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => crate::world::jobs::validate_compiled().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates the generated alias tables (`archive/data/client/aliases.ron`) and
/// returns the row counts.
///
/// The tables (job skill-tree fallbacks, monster sprite aliases, skill icon
/// aliases) are content data extracted from the client archives and the rAthena
/// server tree; see `tools/extract_alias_facts.py` / `tools/gen_aliases.py` and
/// the [`crate::world::library`] alias module. Without a path the compiled
/// fallback is validated; with a path that data file is validated (for example
/// an archive override of `data\client\aliases.ron`).
pub fn check_aliases(path: Option<&std::path::Path>) -> Result<crate::world::AliasesSummary, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            crate::world::validate_alias_data(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => crate::world::validate_compiled_aliases().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates the data-driven key binding table (`data\client\keybinds.ron`)
/// and returns the number of bindings.
///
/// `path` selects a concrete data file (for example an archive override);
/// without a path the compiled fallback is validated. The client binary exposes
/// it as `ro-next --check-keybinds [PATH]`.
pub fn check_keybinds(path: Option<&std::path::Path>) -> Result<usize, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            crate::input::keybinds::validate(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => crate::input::keybinds::validate_compiled().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates the data-driven paper-doll slot table
/// (`data\client\paper_doll_slots.ron`) and returns the number of equipment
/// slots.
///
/// `path` selects a concrete data file (for example an archive override);
/// without a path the compiled fallback is validated. The client binary exposes
/// it as `ro-next --check-paper-doll [PATH]`.
pub fn check_paper_doll(path: Option<&std::path::Path>) -> Result<usize, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            crate::interface::components::paper_doll_slots::validate(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => crate::interface::components::paper_doll_slots::validate_compiled().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates the data-driven skill behaviour and procedural effect data
/// (`archive/data/client/skills.ron`) and returns the row counts.
///
/// `path` selects a concrete data file (for example an archive override);
/// without a path the compiled fallback is validated. The client binary
/// exposes it as `ro-next --check-skills [PATH]`.
pub fn check_skills(path: Option<&std::path::Path>) -> Result<crate::world::SkillsSummary, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            crate::world::validate_skill_data(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => crate::world::validate_compiled_skills().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates the data-driven world/login configuration
/// (`archive/data/client/world_config.ron`) and returns its summary.
///
/// `path` selects a concrete data file (for example an archive override);
/// without a path the compiled fallback is validated. The client binary
/// exposes it as `ro-next --check-world [PATH]`.
pub fn check_world_config(path: Option<&std::path::Path>) -> Result<crate::client_config::WorldConfigSummary, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            crate::client_config::validate_world_config(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => crate::client_config::validate_compiled_world_config().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates the data-driven UI audio paths
/// (`archive/data/client/ui_audio.ron`) and returns their summary.
///
/// `path` selects a concrete data file (for example an archive override);
/// without a path the compiled fallback is validated. The client binary
/// exposes it as `ro-next --check-ui-audio [PATH]`.
pub fn check_ui_audio(path: Option<&std::path::Path>) -> Result<crate::client_config::UiAudioSummary, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            crate::client_config::validate_ui_audio(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => crate::client_config::validate_compiled_ui_audio().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates the data-driven debug map warp list
/// (`archive/data/client/debug_maps.ron`) and returns its number of warps.
///
/// `path` selects a concrete data file (for example an archive override);
/// without a path the compiled fallback is validated. The client binary
/// exposes it as `ro-next --check-debug-maps [PATH]`. This is a debug-only
/// convenience list, not user-facing content.
pub fn check_debug_maps(path: Option<&std::path::Path>) -> Result<usize, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            crate::client_config::validate_debug_maps(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => crate::client_config::validate_compiled_debug_maps().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates every job/body/hair/palette/headgear sprite of the job registry
/// against the real game archives.
pub fn check_appearance() -> Result<crate::world::AppearanceAudit, String> {
    let game_file_loader = GameFileLoader::default();
    game_file_loader.load_archives_from_settings();
    let lua_archive_existed = std::path::Path::new("lua_files.7z").exists();
    game_file_loader.load_patched_lua_files();
    let report = crate::world::jobs::check_appearance(&game_file_loader);
    if !lua_archive_existed {
        game_file_loader.remove_patched_lua_files();
    }
    report
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

    /// Status effects with a persistent glow
    /// ([`crate::world::status_aura_spec`]) that currently have an aura
    /// effect attached, keyed by entity and status id. The value is the
    /// shared "still active" flag (cleared when the status ends, which
    /// removes the effect on its next update) and the last reconciliation
    /// frame seen by [`Self::update_status_auras`].
    status_aura_flags: HashMap<(EntityId, u16), (Arc<AtomicBool>, u64)>,
    /// Frame counter of the status aura reconciliation (used to detect entries
    /// whose status is no longer active without allocating a set per frame).
    status_aura_frame: u64,
    /// Active looping/ground status visuals, keyed by (entity, status). The
    /// value is the shared "still active" flag and the last reconciliation
    /// frame, exactly like `status_aura_flags`.
    status_visual_flags: HashMap<(ragnarok_packets::EntityId, u16), (std::sync::Arc<std::sync::atomic::AtomicBool>, u64)>,
    status_visual_frame: u64,

    /// Skill sounds that were already played at the start of an *instant* skill
    /// Decides when a skill sound plays: instant casts wait for their
    /// completion/damage packet instead of playing a pre-cast sound when the
    /// cast is only acknowledged, and duplicate completion packets are
    /// collapsed (see [`SkillSoundGate`]).
    skill_sound_gate: SkillSoundGate,

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
    /// Lazily loaded flying-projectile sprite/action pairs, keyed by the
    /// projectile kind from the weapon data file. `None` marks a projectile
    /// whose sprites are missing, so the failed load is not retried per hit.
    projectile_assets: HashMap<crate::world::weapon::ProjectileKind, Option<Arc<ArrowAssets>>>,
    point_light_manager: PointLightManager,
    effect_holder: EffectHolder,
    chat_bubbles: ChatBubbles,
    /// Official status icon textures drawn above entity heads (`opt1`/`opt2`).
    head_marks: HeadMarks,
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
    /// Active map weather (`ZC_NOTIFY_EFFECT2` snow/fog/cloud/...).
    weather: Option<crate::world::Weather>,
    /// Classic spirit-sphere texture (`effect\thunder_center.bmp`), drawn as
    /// orbiting blue-white balls around entities with active spirit spheres.
    spirit_sphere_texture: Option<Arc<Texture>>,
    /// Active warp-portal skill units: (unit entity id, world position, spawn
    /// time, expiry). The expiry is a safety net in case the server's
    /// `ZC_SKILL_DISAPPEAR` is lost; the normal removal goes through
    /// `RemoveSkillUnit`. Rendered as the flat ground portal of the original
    /// client.
    warp_skill_units: Vec<(EntityId, Point3<f32>, Instant, Instant)>,
    /// Sound of the entity teleport/logout disappear effect (data-driven, see
    /// `client_config::ui_audio`; `None` disables it).
    teleport_vanish_sound_effect: Option<SoundEffectKey>,
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

    /// Interface/button click sound (data-driven; `None` disables it).
    main_menu_click_sound_effect: Option<SoundEffectKey>,

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
    /// Set through `RO_NEXT_FORCE_UNFOCUSED`/`RO_NEXT_UNFOCUSED_ALWAYS`.
    /// Forces the background frame rate limit, which is useful for headless
    /// verification clients.
    force_background: bool,
    /// The frame rate limit that is currently applied to the graphics engine.
    /// `0` means unlimited.
    active_frames_per_second_limit: u32,
    /// When the effective frame rate should be logged next. Only set when
    /// frame rate logging is enabled through `RO_NEXT_FPS_LOG`.
    frames_per_second_log_deadline: Option<Instant>,
    /// When the next layout/text cache performance line should be logged. Only
    /// set when profiling logging is enabled through `RO_NEXT_PERF_LOG`.
    performance_log_deadline: Option<Instant>,
    /// Set when UI state was changed while the interface frame was alive (where
    /// `self.interface` cannot be borrowed again). Applied at the start of the
    /// next frame.
    interface_layout_invalidation_pending: bool,

    map: Option<Arc<Map>>,
    client_state: State<ClientState>,
}

/// Whether an incoming `ChangeMap` packet with the same map name is a
/// same-map teleport (fly wing, `AL_TELEPORT`, ...) that can be applied without
/// loading the map data again.
///
/// This must be false for a fresh character map entry: after a logout the
/// client keeps the name of the last map (for the "same map" teleport check)
/// but has no loaded map and is in `MapLoading`. Treating that case as a
/// teleport used to skip the map load and left the client on a black screen
/// until the map entry watchdog fired.
fn is_same_map_teleport(
    current_map_name: Option<&str>,
    incoming_map_name: &str,
    has_loaded_map: bool,
    session_state: SessionState,
) -> bool {
    current_map_name == Some(incoming_map_name) && has_loaded_map && session_state == SessionState::InGame
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

        // Check if ro-next is in the correct working directory and if not, try to
        // correct it.
        // NOTE: This check might be temporary or feature gated in the future.
        time_phase!("adjust working directory", {
            if !std::fs::metadata("archive").is_ok_and(|metadata| metadata.is_dir()) {
                #[cfg(feature = "debug")]
                print_debug!(
                    "[{}] failed to find archive directory, attempting to change working directory {}",
                    "warning".yellow(),
                    "ro-next".magenta()
                );

                if let Err(_error) = std::env::set_current_dir("ro-next") {
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

        // The protocol result-code texts are data as well (see the
        // `ro-next-messages` crate); an archive override wins over the
        // compiled fallback.
        time_phase!("load protocol messages", {
            load_protocol_messages(game_file_loader.as_ref());
        });

        // Client configuration data (default/login map, start-camera focus,
        // quest-icon colours and UI audio paths). Each file is loaded through
        // the archive with the compiled fallback, so a server can override it
        // without a rebuild.
        time_phase!("load client config", {
            crate::client_config::load(game_file_loader.as_ref());
        });

        // The data-driven keyboard bindings (window shortcuts, hotbar/emotion
        // keys, confirm keys); an archive override wins over the compiled
        // fallback, see `input::keybinds`.
        time_phase!("load keybinds", {
            crate::input::keybinds::load(game_file_loader.as_ref());
        });

        // The data-driven paper-doll slot layout (equipment window); an archive
        // override wins over the compiled fallback, see
        // `interface::components::paper_doll_slots`.
        time_phase!("load paper doll slots", {
            crate::interface::components::paper_doll_slots::load(game_file_loader.as_ref());
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
                game_file_loader.clone(),
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
            let weather = crate::world::Weather::new(&texture_loader);
        });

        time_phase!("load spirit sphere assets", {
            let spirit_sphere_texture = texture_loader.get_or_load("effect\\thunder_center.bmp", ImageType::Color).ok();
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
            start_camera.set_focus_point(crate::client_config::start_camera_focus());
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

            // The official sprite damage font (loose folder in the client
            // archive). Without it the damage numbers fall back to text.
            crate::world::damage_font::install(crate::world::damage_font::DamageFont::load(&sprite_loader, &action_loader));

            let mut particle_holder = ParticleHolder::default();
            // Arrow impact sounds are played from the particle update, so the
            // holder needs the audio engine (`_hit_arrow.wav`).
            particle_holder.install_audio(audio_engine.clone());
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

            // Data-driven UI audio paths (archive override of
            // `data\client\ui_audio.ron`, see `client_config::ui_audio`).
            let main_menu_click_sound_effect = crate::client_config::main_menu_click_sound().map(|path| audio_engine.load(path));
            let teleport_vanish_sound_effect = crate::client_config::teleport_vanish_sound().map(|path| audio_engine.load(path));
        });

        time_phase!("load default map", {
            let map = map_loader
                .load(
                    crate::client_config::default_map().to_string(),
                    &model_loader,
                    texture_loader.clone(),
                    video_loader,
                    &library,
                )
                .expect("failed to load initial map");

            directional_shadow_camera.set_level_bound(map.get_level_bound());

            audio_engine.play_background_music_track(crate::client_config::background_music());
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

        crate::input::keybinds::load_user_bindings();

        interface.open_window(LoginWindow::new(
            crate::client_state().login_window(),
            crate::client_state().login_settings(),
            crate::client_state().client_info(),
        ));

        // Official status icon textures for the head mark overlay (opt1/opt2).
        let head_marks = HeadMarks::new(&texture_loader);

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
            skill_sound_gate: SkillSoundGate::default(),
            status_aura_flags: HashMap::new(),
            status_aura_frame: 0,
            status_visual_flags: HashMap::new(),
            status_visual_frame: 0,
            map_window_cycle: MapWindowCycle::Closed,
            minimap_last_tile: None,
            particle_holder,
            projectile_assets: HashMap::new(),
            point_light_manager,
            effect_holder,
            chat_bubbles: ChatBubbles::default(),
            head_marks,
            pending_item_drag: None,
            skill_name_texts: SkillNameTexts::default(),
            cast_bars: CastBars::default(),
            skill_completion_tracker: SkillCompletionTracker::default(),
            aftercast_act_until: None,
            warp_portal,
            weather,
            spirit_sphere_texture,
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
                    "[display] hint: use RO_NEXT_X11=1 to force X11/XWayland or RO_NEXT_WAYLAND=1 to force native Wayland"
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
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
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
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
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
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
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

    /// Updates every part of the client that depends on the size of the window.
    ///
    /// This is called for every resize event, but also once after the window
    /// was created. The window is created with the configured resolution,
    /// which can differ from [`INITIAL_SCREEN_SIZE`], and winit does not
    /// necessarily send a resize event for the initial size. Without this
    /// the interface and all renderers would keep using the initial size,
    /// which stretches the interface and breaks its hit testing.
    /// Request the names of all entities that should show a name label.
    ///
    /// Entity details (the name) are loaded lazily from the map server and were
    /// previously only requested for the entity under the mouse. That made the
    /// always on name labels only appear for entities that had been hovered
    /// once before.
    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
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
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
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

    /// Shows the aggregated experience gain in the chat once the packet burst
    /// is over.
    ///
    /// The line and its labels are localization templates
    /// (`chat_experience_gained` + `chat_experience_*`), rendered with the same
    /// `{name}` placeholders as the other client feedback.
    fn flush_experience_feedback(&mut self) {
        let Some(last_packet) = self.pending_experience.last_packet else {
            return;
        };

        if last_packet.elapsed() < EXPERIENCE_FEEDBACK_DELAY {
            return;
        }

        let Some(gains) = self.pending_experience.take_gains() else {
            return;
        };

        let localization = self.client_state.follow(client_state().localization());
        let labels = localization.experience_labels();
        let mut parts = Vec::new();

        for (amount, label) in gains.into_iter().zip(labels) {
            if amount != 0 {
                parts.push(format!(
                    "{}{} {}",
                    if amount < 0 { "-" } else { "" },
                    format_amount(amount.unsigned_abs()),
                    label
                ));
            }
        }

        let template = self.client_state.follow(client_state().localization().chat_experience_gained());
        let message = ro_next_messages::render_template(template, &[("gains", &parts.join(", "))]);

        self.push_chat_information(message);
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

    /// Plays the official sound of a skill once its completion (or damage)
    /// packet arrives.
    ///
    /// The play timing is decided by [`SkillSoundGate`]; this only forwards the
    /// audible part so the gate stays unit-testable without audio.
    fn play_completion_sound(
        &mut self,
        entity_id: EntityId,
        skill_id: SkillId,
        phase: SkillPhase,
        position: Option<cgmath::Point3<f32>>,
        client_tick: ClientTick,
    ) {
        if self.skill_sound_gate.claim_completion(entity_id, skill_id, client_tick) {
            self.skill_sounds
                .play_skill_sound_for_phase(skill_id, phase, Some(entity_id), position);
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
        // casting (skill flag + job capability, both data). Unknown jobs
        // default to "blocked".
        if can_move_while_casting(
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

    /// Spawns, keeps alive and removes the persistent glow effects of active
    /// status effects (the Assumptio glow and the documented buff glows, see
    /// [`crate::world::status_aura_spec`]).
    ///
    /// The active statuses are reconciled every frame instead of being wired to
    /// the status packets: status start/refresh/end, local timer expiry, death,
    /// warp and entities entering/leaving the view then all behave the same,
    /// and adding another glowing status only needs a data row in
    /// `archive/data/client/status_effects.ron` (see
    /// `tools/data/status_effect_custom.ron`).
    ///
    /// The reconciliation itself does not allocate: the per-frame active set is
    /// encoded as the last-seen frame next to each spawned aura, so only the
    /// (rare) spawn path touches the allocator.
    fn update_status_auras(&mut self) {
        self.status_aura_frame = self.status_aura_frame.wrapping_add(1);
        let frame = self.status_aura_frame;

        let statuses = self.client_state.follow(client_state().status_effects());
        let entities = self.client_state.follow(client_state().entities());

        for status in statuses.effects() {
            let status_index = status.status_index();
            if crate::world::status_aura_spec(status_index).is_none() {
                continue;
            }

            let Some(entity) = entities.iter().find(|entity| entity.get_entity_id().0 == status.entity_id()) else {
                // The entity left the view; the server re-sends its statuses
                // with `ZC_EFST_SET_ENTER` when it comes back.
                continue;
            };
            let entity_id = entity.get_entity_id();

            if entity.is_dead() || entity.is_fading() {
                continue;
            }

            let key = (entity_id, status_index);
            if let Some((_, last_seen)) = self.status_aura_flags.get_mut(&key) {
                *last_seen = frame;
                continue;
            }

            let Some(spec) = crate::world::status_aura_spec(status_index) else {
                continue;
            };
            let is_active = Arc::new(AtomicBool::new(true));

            let Some(aura) = crate::world::StatusAura::new(&self.texture_loader, entity_id, spec, is_active.clone()) else {
                crate::logging::log_warn!("status aura: failed to load the glow textures of status {}", status_index);
                // Do not retry every frame if the textures are missing.
                self.status_aura_flags.insert(key, (Arc::new(AtomicBool::new(false)), frame));
                continue;
            };

            if self.effect_holder.add_unit(Box::new(aura), entity_id) {
                crate::logging::log_debug!(
                    "status aura: spawned glow for status {} on entity {}",
                    status_index,
                    entity_id.0
                );
                self.status_aura_flags.insert(key, (is_active, frame));
            }
            // If the effect budget rejected the aura, do not record it: the
            // spawn is retried on the next frame, when the budget may have
            // room.
        }

        // Deactivate auras whose status ended (or whose entity is no longer
        // eligible); the effect removes itself on its next update.
        self.status_aura_flags.retain(|_, (is_active, last_seen)| {
            if *last_seen == frame {
                return true;
            }

            is_active.store(false, Ordering::Relaxed);
            false
        });
    }

    /// Reconciles the looping/ground status visuals with the active statuses.
    fn update_status_visuals(&mut self) {
        use crate::world::StatusVisualKind;

        self.status_visual_frame = self.status_visual_frame.wrapping_add(1);
        let frame = self.status_visual_frame;

        let statuses = self.client_state.follow(client_state().status_effects());
        let entities = self.client_state.follow(client_state().entities());

        for status in statuses.effects() {
            let status_index = status.status_index();
            let Some(visual) = crate::world::status_visual(status_index) else {
                continue;
            };
            if !matches!(
                visual.kind,
                StatusVisualKind::LoopingEffect { .. } | StatusVisualKind::GroundRing { .. }
            ) {
                continue;
            }
            let Some(entity) = entities.iter().find(|entity| entity.get_entity_id().0 == status.entity_id()) else {
                continue;
            };
            if entity.is_dead() || entity.is_fading() {
                continue;
            }
            let entity_id = entity.get_entity_id();
            let key = (entity_id, status_index);
            if let Some((_, last_seen)) = self.status_visual_flags.get_mut(&key) {
                *last_seen = frame;
                continue;
            }

            let active = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
            let spawned = match visual.kind {
                StatusVisualKind::LoopingEffect { file, anchor, scale, .. } => {
                    let Ok(effect) = self.effect_loader.get_or_load(file, &self.texture_loader) else {
                        crate::logging::log_warn!("status visual: failed to load looping effect '{}'", file);
                        self.status_visual_flags
                            .insert(key, (std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)), frame));
                        continue;
                    };
                    let offset = match anchor {
                        crate::world::StatusVisualAnchor::Body => cgmath::Vector3::new(0.0, 0.0, 0.0),
                        crate::world::StatusVisualAnchor::Overhead => cgmath::Vector3::new(0.0, 20.0, 0.0),
                        crate::world::StatusVisualAnchor::Ground => cgmath::Vector3::new(0.0, 1.0, 0.0),
                    };
                    let frame_timer = effect.new_frame_timer();
                    let inner = crate::world::EffectWithLight::new(
                        effect,
                        frame_timer,
                        crate::world::EffectCenter::Entity(entity_id, cgmath::Point3::new(0.0, 0.0, 0.0)),
                        offset,
                        scale,
                        crate::world::PointLightId::new(entity_id.0.wrapping_add(status_index as u32).wrapping_add(0x5000_0000)),
                        cgmath::Vector3::new(0.0, 20.0, 0.0),
                        crate::graphics::Color::WHITE,
                        0.0,
                        true,
                    );
                    self.effect_holder.add_unit(
                        Box::new(crate::world::StatusLoopingEffect::new(inner, active.clone())),
                        entity_id,
                    )
                }
                StatusVisualKind::GroundRing {
                    texture,
                    radius,
                    height,
                    color,
                    pulse,
                    ..
                } => crate::world::StatusGroundRing::new(
                    &self.texture_loader,
                    entity_id,
                    texture,
                    active.clone(),
                    radius,
                    height,
                    color,
                    pulse,
                )
                .map(|ring| self.effect_holder.add_unit(Box::new(ring), entity_id))
                .unwrap_or(false),
                _ => false,
            };

            if spawned {
                self.status_visual_flags.insert(key, (active, frame));
            }
        }

        self.status_visual_flags.retain(|_, (active, last_seen)| {
            if *last_seen == frame {
                return true;
            }
            active.store(false, std::sync::atomic::Ordering::Relaxed);
            false
        });
    }

    /// Updates the local status effect timers and shows/hides the buff bar.
    fn update_status_effects(&mut self, client_tick: ClientTick) {
        self.client_state
            .follow_mut(client_state().notifications())
            .retain_alive(client_tick.0);
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

    /// Recomputes the best status sprite visual of `entity_id` and applies it.
    ///
    /// The data-driven table is queried for every active status of the entity:
    /// the highest-priority `BodyTint` wins (it also carries the additive
    /// body/weapon scope), the first `BodyFade` is applied, and `WeaponBlink`
    /// is OR-ed. When no visual matches, the previous state is cleared so
    /// ending a status cannot leave a stale colour or blend behind.
    fn refresh_entity_status_visual(&mut self, entity_id: ragnarok_packets::EntityId) {
        let state = {
            let statuses = self.client_state.follow(client_state().status_effects());
            crate::world::resolve_status_visual_state(
                statuses
                    .effects()
                    .iter()
                    .filter(|effect| effect.entity_id() == entity_id.0)
                    .filter_map(|effect| crate::world::status_visual(effect.status_index()))
                    .map(|visual| &visual.kind),
            )
        };

        if let Some(entity) = self
            .client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id() == entity_id)
        {
            entity.set_status_tint(state.tint);
            entity.set_status_alpha(state.alpha);
            entity.set_weapon_blink(state.weapon_blink);
            entity.set_status_additive(state.body_additive, state.weapon_additive);
        }
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
            SkillType::Passive => self.push_localized_chat(client_state().localization().chat_skill_passive(), &[(
                "skill",
                &skill_id.0.to_string(),
            )]),
            SkillType::SelfCast => self.cast_self_skill(skill_id, level),
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

    /// Casts a self-cast skill at the local player.
    ///
    /// Channeling skills (Rolling Cutter 2036; data flag in `skills.ron`) use
    /// the channeling packet path, every other self cast uses the ordinary
    /// cast packet. The skill id list is data, not a branch at the call site.
    fn cast_self_skill(&mut self, skill_id: SkillId, level: SkillLevel) {
        let entity_id = self.client_state.follow(this_entity().manually_asserted()).get_entity_id();

        if crate::world::skills::channels(skill_id) {
            let _ = self.networking_system.cast_channeling_skill(skill_id, level, entity_id);
        } else {
            let _ = self.networking_system.cast_skill(skill_id, level, entity_id);
        }
    }

    #[cfg(feature = "debug")]
    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
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
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
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
                LoaderId::AnimationData(_)
                    | LoaderId::FalconAnimationData(_)
                    | LoaderId::WeaponAnimationData(_)
                    | LoaderId::ShieldAnimationData(_)
                    | LoaderId::RobeAnimationData(_)
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
                (LoaderId::CharacterPreview(slot), LoadableResource::AnimationData(animation_data)) => {
                    self.client_state
                        .follow_mut(client_state().character_previews())
                        .set_ready(slot, animation_data);
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
                (LoaderId::ImfData(entity_id, ..), LoadableResource::Imf(imf)) => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_imf(imf);
                    }
                }
                (LoaderId::CartAnimationData(entity_id), LoadableResource::CartAnimation(cart_animation)) => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_cart_animation(cart_animation);
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
                (LoaderId::ShieldAnimationData(entity_id), LoadableResource::ShieldAnimation(shield_animation)) => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_shield_animation(shield_animation);
                    }
                }
                (LoaderId::RobeAnimationData(entity_id), LoadableResource::RobeAnimation(robe_animation)) => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_robe_animation(robe_animation);
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
                            self.audio_engine
                                .play_background_music_track(crate::client_config::background_music());
                            self.directional_shadow_camera.set_level_bound(map.get_level_bound());

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
                                self.open_character_selection_window();
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

                            self.start_camera.set_focus_point(crate::client_config::start_camera_focus());
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
                            self.on_navigation_map_loaded(&map_name);
                            let _ = self.networking_system.map_loaded();
                        }
                    }
                }
                (LoaderId::SkillSprite(skill_id), LoadableResource::SkillSprite { sprite }) => {
                    self.client_state
                        .follow_mut(client_state().hotbar())
                        .update_skill_resource(skill_id, |skill| skill.sprite = Some(sprite.clone()));

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
                        .follow_mut(client_state().hotbar())
                        .update_skill_resource(skill_id, |skill| skill.actions = Some(actions.clone()));

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
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    /// Reacts to an asynchronous load that failed.
    ///
    /// A failed map load is a dead end for the current map entry: the client
    /// has no map to render and would stay on a black screen forever. In
    /// that case the map connection is dropped, which sends the player back
    /// to the character selection screen (see
    /// `NetworkEvent::MapServerDisconnected`), and the error is shown
    /// there.
    /// Keep the character selection previews in sync with the character
    /// slots.
    ///
    /// Called every frame while the window is open. For every visible occupied
    /// slot whose preview is not loaded yet, the composed idle animation of the
    /// character is requested from the async loader; the result is applied by
    /// [`Self::update_loaded_resources`]. When the window is not open the
    /// loaded previews are dropped, so returning to the login screen does not
    /// keep character sprites alive.
    /// Repaint the character selection window while the anti-crash selection
    /// cooldown is running, so the countdown text updates and the cards become
    /// selectable again once the cooldown expired (the click handlers are only
    /// registered during a layout pass).
    fn update_character_selection_cooldown(&mut self) {
        if !self.interface.is_window_with_class_open(WindowClass::CharacterSelection) {
            return;
        }

        if self
            .client_state
            .follow(client_state().character_selection_cooldown())
            .is_repainting(Instant::now())
        {
            self.interface.invalidate_layouts();
        }
    }

    fn update_character_selection_previews(&mut self) {
        if !self.interface.is_window_with_class_open(WindowClass::CharacterSelection) {
            let previews = self.client_state.follow_mut(client_state().character_previews());
            if previews.slot_count() > 0 {
                previews.clear();
            }
            return;
        }

        let reported_slots = self.client_state.follow(client_state().character_slots()).get_slot_count();
        let slot_count = visible_character_slots(reported_slots);
        let previous_slot_count = self.client_state.follow(client_state().character_previews()).slot_count();

        self.client_state
            .follow_mut(client_state().character_previews())
            .set_slot_count(slot_count);

        // Log the clamp once per character list, not every frame the window is
        // open.
        if reported_slots > MAX_CHARACTER_SLOTS && previous_slot_count != slot_count {
            crate::logging::log_debug!(
                "character selection: server reports {reported_slots} slots, showing the first {MAX_CHARACTER_SLOTS}"
            );
        }

        for slot in 0..slot_count {
            // The character list can change while the window is open (delete,
            // slot switch); clone the row so the state borrow ends before the
            // loader is called.
            let character_information = self
                .client_state
                .follow(client_state().character_slots())
                .character_in_slot(slot)
                .cloned();

            let Some(character_information) = character_information else {
                self.client_state.follow_mut(client_state().character_previews()).clear_slot(slot);
                continue;
            };

            let character_id = character_information.character_id;
            let is_current = self
                .client_state
                .follow(client_state().character_previews())
                .is_current(slot, character_id);

            if is_current {
                continue;
            }

            let entity_part_files = character_preview_part_files(&self.library, &character_information);

            self.client_state
                .follow_mut(client_state().character_previews())
                .request(slot, character_id);

            // The appearance may already be cached (for example from a previous
            // visit of the character selection screen): in that case no
            // asynchronous load happens and the data can be applied right away.
            if let Some(animation_data) = self.async_loader.request_character_preview_load(slot, entity_part_files) {
                self.client_state
                    .follow_mut(client_state().character_previews())
                    .set_ready(slot, animation_data);
            }
        }
    }

    fn handle_failed_load(&mut self, id: LoaderId, error: LoadError) {
        // A missing preview sprite must not be requested again every frame: the
        // slot keeps its name/level and simply shows no character.
        if let LoaderId::CharacterPreview(slot) = &id {
            crate::logging::log_debug!("character preview unavailable for slot {slot}: {error:?}");
            self.client_state
                .follow_mut(client_state().character_previews())
                .set_unavailable(*slot);
            return;
        }

        // Missing/placeholder weapon sprites are expected in this data set (the
        // CondaRO archives have no drawable per-job weapon clips; see
        // `world::weapon`); the async loader caches the failure and the entity is
        // rendered without the weapon layer. Keep this out of the generic asset
        // failure reporting (and out of stderr) so map entry does not spam the
        // user or the log for every population-engine shell.
        if let LoaderId::WeaponAnimationData(entity_id)
        | LoaderId::ShieldAnimationData(entity_id)
        | LoaderId::RobeAnimationData(entity_id) = &id
        {
            crate::logging::log_debug!("equipment animation unavailable for entity {}: {error:?}", entity_id.0);
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

                let message = self.render_localized(client_state().localization().error_map_data_missing(), &[(
                    "map_name",
                    map_name.as_str(),
                )]);

                self.abort_map_entry(message);
            }
            AssetFailureSeverity::Transient => {
                if matches!(
                    id,
                    LoaderId::AnimationData(_)
                        | LoaderId::FalconAnimationData(_)
                        | LoaderId::WeaponAnimationData(_)
                        | LoaderId::ShieldAnimationData(_)
                        | LoaderId::RobeAnimationData(_)
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
    /// Closes the trade windows and drops the transient trade state.
    fn close_trade(&mut self) {
        self.interface.close_window_with_class(WindowClass::Trade);
        self.interface.close_window_with_class(WindowClass::TradeRequest);
        self.client_state.follow_mut(client_state().trade()).close();
    }

    fn close_storage_window(&mut self) {
        self.interface.close_window_with_class(WindowClass::Storage);
        self.client_state.follow_mut(client_state().storage()).clear();
        *self.client_state.follow_mut(client_state().storage_amount()) = 0;
        *self.client_state.follow_mut(client_state().storage_maximum_amount()) = 0;
        *self.client_state.follow_mut(client_state().storage_name()) = String::new();
    }

    /// Account id of the logged-in player. The banking requests are checked
    /// against it by the server (`clif_parse_BankOpen`, clif.cpp:7342).
    fn own_account_id(&self) -> Option<AccountId> {
        self.saved_login_data.map(|login_data| login_data.account_id)
    }

    /// Open the bank window, start the banking session and request the vault.
    fn open_bank_window(&mut self) {
        let Some(account_id) = self.own_account_id() else {
            return;
        };

        if !self.interface.is_window_with_class_open(WindowClass::Bank) {
            self.interface.open_window(BankWindow);
        }

        self.client_state.follow_mut(client_state().bank()).open();

        let _ = self.networking_system.open_bank(account_id);
        let _ = self.networking_system.check_bank(account_id);
    }

    /// Close the bank window and end the banking session.
    fn close_bank_window(&mut self) {
        if let Some(account_id) = self.own_account_id() {
            let _ = self.networking_system.close_bank(account_id);
        }

        self.client_state.follow_mut(client_state().bank()).close();
        self.interface.close_window_with_class(WindowClass::Bank);
    }

    /// Close the cart window and drop the transient cart list.
    ///
    /// rAthena sends `ZC_CARTOFF` when the cart is removed, and a map change
    /// invalidates the old list as well.
    fn close_cart_window(&mut self) {
        self.interface.close_window_with_class(WindowClass::Cart);
        self.client_state.follow_mut(client_state().cart()).clear();
    }

    /// Open the shared item service window for the given service.
    ///
    /// The window is opened whenever the server sends a service list; opening
    /// it again for a different service replaces the previous window because
    /// all four services share one window class.
    fn open_item_service_window(&mut self, kind: ItemServiceKind) {
        self.interface.close_window_with_class(WindowClass::ItemService);
        self.interface.open_window(ItemServiceWindow::new(kind));
    }

    /// Close the item service window and drop its list.
    fn close_item_service_window(&mut self) {
        self.interface.close_window_with_class(WindowClass::ItemService);
        self.client_state.follow_mut(client_state().item_service()).clear();
    }

    /// Open the NPC refine UI window in the given step.
    fn open_refining_window(&mut self, mode: RefiningMode) {
        self.interface.close_window_with_class(WindowClass::Refining);
        self.interface.open_window(RefiningWindow::new(mode));
    }

    /// Close the NPC refine UI and drop its state.
    fn close_refining_window(&mut self) {
        self.interface.close_window_with_class(WindowClass::Refining);
        self.client_state.follow_mut(client_state().item_service().refining()).clear();
    }

    /// Equipment that the NPC refine UI offers for selection.
    ///
    /// rAthena's `clif_refineui_info` requires the item to be identified and
    /// undamaged (`clif.cpp:22520`); the refine database decides whether the
    /// item is refinable at all, so the client only filters by the data it has
    /// (weapons/armor, undamaged, identified, unequipped and below the
    /// pre-renewal refine maximum of 10, `status.hpp:36`).
    fn refinable_inventory_items(&self) -> Vec<ItemServiceEntry> {
        self.client_state
            .follow(client_state().inventory().items())
            .iter()
            .filter(|item| {
                if !item.is_identified() || item.is_equipped() {
                    return false;
                }

                if item.item_type != item_type::WEAPON && item.item_type != item_type::ARMOR {
                    return false;
                }

                match &item.details {
                    ro_next_networking::InventoryItemDetails::Equippable {
                        refinement_level, flags, ..
                    } => *refinement_level < 10 && !flags.contains(EquippableItemFlags::IS_BROKEN),
                    ro_next_networking::InventoryItemDetails::Regular { .. } => false,
                }
            })
            .map(|item| {
                let refine = match &item.details {
                    ro_next_networking::InventoryItemDetails::Equippable { refinement_level, .. } => *refinement_level,
                    ro_next_networking::InventoryItemDetails::Regular { .. } => 0,
                };
                ItemServiceEntry::new(item.index, item.item_id, item.metadata.name.clone(), refine)
            })
            .collect()
    }

    /// Resolve a service row for one of the player's own inventory items.
    fn inventory_service_entry(&self, index: InventoryIndex) -> Option<ItemServiceEntry> {
        let inventory = self.client_state.follow(client_state().inventory().items());
        let item = inventory.iter().find(|item| item.index == index)?;
        let refine = match &item.details {
            ro_next_networking::InventoryItemDetails::Equippable { refinement_level, .. } => *refinement_level,
            ro_next_networking::InventoryItemDetails::Regular { .. } => 0,
        };

        Some(ItemServiceEntry::new(index, item.item_id, item.metadata.name.clone(), refine))
    }

    /// Close the vending browse window and drop the shop list of the player's
    /// shop.
    fn close_vending_browse(&mut self) {
        self.interface.close_window_with_class(WindowClass::VendingBrowse);
        self.client_state.follow_mut(client_state().vending_browse()).close();
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
    /// Open the character selection window, centred in the available screen
    /// area, and start the anti-crash selection cooldown if the client just
    /// returned from a map session.
    fn open_character_selection_if_closed(&mut self) {
        if !self.interface.is_window_with_class_open(WindowClass::CharacterSelection) {
            #[cfg(not(feature = "debug"))]
            self.interface.close_all_windows();

            #[cfg(feature = "debug")]
            self.interface.close_all_windows_except(DEBUG_WINDOWS);

            self.open_character_selection_window();
        }
    }

    fn open_character_selection_window(&mut self) {
        self.client_state
            .follow_mut(client_state().character_selection_cooldown())
            .start(Instant::now());

        let window_position = {
            let state = self.client_state.follow(client_state());
            character_selection_window_position(state)
        };

        self.interface.open_window_at_auto_size(
            CharacterSelectionWindow::new(
                client_state().character_slots(),
                client_state().switch_request(),
                client_state().character_previews(),
                client_state().character_selection_cooldown(),
            ),
            window_position,
        );
    }

    fn return_to_character_selection(&mut self) {
        // The server may still be cleaning up the old map session. Block rapid
        // re-selection until the character selection window is shown and the
        // grace period started.
        self.client_state.follow_mut(client_state().character_selection_cooldown()).arm();
        // Chat rooms and the pincode prompt belong to the previous map/account
        // session; never carry them into the character selection screen.
        self.client_state
            .follow_mut(client_state().content_windows().chat_room())
            .leave_locally();
        self.client_state.follow_mut(client_state().content_windows().pincode()).dismiss();

        self.map = None;
        // The same-map teleport fast path must never apply to the next character
        // map entry: the client is back at the character selection and the map
        // is unloaded. Keeping the old name used to skip the map load for a
        // character that lives on the same map as the menu background.
        self.current_map_name = None;
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
        self.audio_engine
            .play_background_music_track(crate::client_config::background_music());

        #[cfg(not(feature = "debug"))]
        self.interface.close_all_windows();

        #[cfg(feature = "debug")]
        self.interface.close_all_windows_except(DEBUG_WINDOWS);

        self.async_loader
            .request_map_load(crate::client_config::default_map().to_string(), Some(TilePosition::new(0, 0)));
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
            if entity.needs_cart_animation() {
                missing.push(LoaderId::CartAnimationData(entity_id));
            }
            if entity.needs_imf() {
                missing.push(LoaderId::ImfData(
                    entity_id,
                    entity.get_job_id(),
                    matches!(entity.get_sex(), ragnarok_packets::Sex::Female),
                ));
            }
            if entity.needs_weapon_animation() {
                missing.push(LoaderId::WeaponAnimationData(entity_id));
            }
            if entity.needs_shield_animation() {
                missing.push(LoaderId::ShieldAnimationData(entity_id));
            }
            if entity.needs_robe_animation() {
                missing.push(LoaderId::RobeAnimationData(entity_id));
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
            LoaderId::AnimationData(entity_id)
            | LoaderId::FalconAnimationData(entity_id)
            | LoaderId::CartAnimationData(entity_id)
            | LoaderId::WeaponAnimationData(entity_id)
            | LoaderId::ShieldAnimationData(entity_id)
            | LoaderId::RobeAnimationData(entity_id) => entity_id,
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
                    entity.has_cart(),
                    entity.get_weapon_part_candidates(),
                    entity.get_shield_part_candidates(),
                    entity.get_robe_part_candidates(),
                    entity.get_cart_part_candidates(),
                    entity.has_weapon_animation(),
                    entity.has_shield_animation(),
                    entity.has_robe_animation(),
                    entity.has_cart_animation(),
                )
            });

        let Some((
            entity_type,
            entity_part_files,
            has_falcon,
            has_cart,
            weapon_candidates,
            shield_candidates,
            robe_candidates,
            cart_candidates,
            has_weapon_animation,
            has_shield_animation,
            has_robe_animation,
            has_cart_animation,
        )) = entity_info
        else {
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
            LoaderId::CartAnimationData(_) => {
                if !has_cart || has_cart_animation {
                    self.asset_retries.note_available(&LoaderId::CartAnimationData(entity_id));
                    return;
                }

                match self.async_loader.request_cart_animation_data_load(entity_id, cart_candidates) {
                    Some(cart_animation) => {
                        if let Some(entity) = self.find_entity_mut(entity_id) {
                            entity.set_cart_animation(cart_animation);
                        }
                        self.asset_retries.note_available(&LoaderId::CartAnimationData(entity_id));
                    }
                    None => self.asset_retries.note_request(LoaderId::CartAnimationData(entity_id), now),
                }
            }
            LoaderId::ImfData(_, job_id, female) => {
                let sex = if female {
                    ragnarok_packets::Sex::Female
                } else {
                    ragnarok_packets::Sex::Male
                };
                match self.async_loader.request_imf_data_load(entity_id, job_id, sex) {
                    Some(imf) => {
                        if let Some(entity) = self.find_entity_mut(entity_id) {
                            entity.set_imf(imf);
                        }
                        self.asset_retries.note_available(&LoaderId::ImfData(entity_id, job_id, female));
                    }
                    None => self.asset_retries.note_request(LoaderId::ImfData(entity_id, job_id, female), now),
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
            LoaderId::ShieldAnimationData(_) => {
                if shield_candidates.is_empty() || has_shield_animation {
                    self.asset_retries.note_available(&LoaderId::ShieldAnimationData(entity_id));
                    return;
                }

                match self.async_loader.request_shield_animation_data_load(entity_id, shield_candidates) {
                    Some(shield_animation) => {
                        if let Some(entity) = self.find_entity_mut(entity_id) {
                            entity.set_shield_animation(shield_animation);
                        }
                        self.asset_retries.note_available(&LoaderId::ShieldAnimationData(entity_id));
                    }
                    None => self.asset_retries.note_request(LoaderId::ShieldAnimationData(entity_id), now),
                }
            }
            LoaderId::RobeAnimationData(_) => {
                if robe_candidates.is_empty() || has_robe_animation {
                    self.asset_retries.note_available(&LoaderId::RobeAnimationData(entity_id));
                    return;
                }

                match self.async_loader.request_robe_animation_data_load(entity_id, robe_candidates) {
                    Some(robe_animation) => {
                        if let Some(entity) = self.find_entity_mut(entity_id) {
                            entity.set_robe_animation(robe_animation);
                        }
                        self.asset_retries.note_available(&LoaderId::RobeAnimationData(entity_id));
                    }
                    None => self.asset_retries.note_request(LoaderId::RobeAnimationData(entity_id), now),
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
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
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
            self.asset_retries.forget(&LoaderId::ShieldAnimationData(entity_id));
            self.asset_retries.forget(&LoaderId::RobeAnimationData(entity_id));
        }

        self.client_state
            .follow_mut(client_state().ground_items())
            .retain(|item| !item.should_be_removed(client_tick));
    }

    /// Whether the local player may attack `entity_id` under the current
    /// map's targeting rules.
    ///
    /// Monsters are attackable on every map (the official client's
    /// `IsMonsterClass` branch of `CGameMode::HandleActorPick`); players are
    /// subject to [`player_attack_decision`]. Entity types that are neither
    /// (NPCs, warps, pets, homunculi, mercenaries) are not attack targets of
    /// the local player's basic attack.
    fn can_attack_entity(&self, entity_id: EntityId) -> bool {
        let Some((entity_type, is_dead)) = self
            .client_state
            .follow(client_state().entities())
            .iter()
            .find(|entity| entity.get_entity_id() == entity_id)
            .map(|entity| (entity.get_entity_type(), entity.is_dead()))
        else {
            return false;
        };

        match entity_type {
            EntityType::Monster => true,
            EntityType::Player => !is_dead && self.can_attack_player(entity_id),
            _ => false,
        }
    }

    /// Whether the local player may attack the player `entity_id`.
    ///
    /// This is the client-side half of the official
    /// `CGameMode::HandleActorPick`/`ProcessActorPick` player branch: the
    /// map's PvP/GvG/PK zone, the target's party/guild relation and the
    /// target's dead state decide whether an attack may be dispatched.
    fn can_attack_player(&self, entity_id: EntityId) -> bool {
        let Some((target_character_id, target_guild_id)) = self
            .client_state
            .follow(client_state().entities())
            .iter()
            .find(|entity| entity.get_entity_id() == entity_id)
            .map(|entity| (entity.get_character_id(), entity.get_guild_id()))
        else {
            return false;
        };

        let Some(local) = self.client_state.try_follow(this_entity()) else {
            return false;
        };

        let local_entity_id = local.get_entity_id();
        let local_guild_id = local.get_guild_id();

        let zone = player_combat_zone(
            *self.client_state.follow(client_state().map_type()),
            *self.client_state.follow(client_state().map_flags()),
        );

        let is_party_member = self
            .client_state
            .follow(client_state().party())
            .is_character_member(target_character_id);

        // `GuildInfo::HasRelatedGuild` covers the own guild and allied
        // guilds; the own guild id from the local entity is used as a
        // fallback when the guild window data has not arrived yet.
        let guild = self.client_state.follow(client_state().guild());
        let is_related_guild = target_guild_id != 0 && (target_guild_id == local_guild_id || guild.is_related_guild(target_guild_id));

        player_attack_decision(zone, TargetRelations {
            is_self: entity_id == local_entity_id,
            is_party_member,
            is_related_guild,
            is_dead: false,
        })
        .is_allowed()
    }

    /// Fire any action that the player buffered while out of range or while
    /// still moving (attack, pick up item). Must be called after
    /// [`Self::update_entities`] so that the player's `stopped_moving` state
    /// reflects this frame.
    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    fn process_buffered_action(&mut self) {
        let Some(true) = self.client_state.try_follow(this_entity()).map(|player| player.stopped_moving()) else {
            return;
        };

        let Some(buffered_action) = self.client_state.follow_mut(client_state().buffered_action()).take() else {
            return;
        };

        match buffered_action {
            BufferedAction::AttackEntity { entity_id } => {
                // Re-check the targeting rule here: the buffered attack can
                // outlive a map change or the target can die while the player
                // walks into range.
                if !self.can_attack_entity(entity_id) {
                    return;
                }

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
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    fn update_audio_engine(&self, current_camera: &dyn Camera) {
        // We set the listener roughly at ear height.
        const EAR_HEIGHT: Vector3<f32> = Vector3::new(0.0, 5.0, 0.0);
        let listener = current_camera.focus_point() + EAR_HEIGHT;

        self.audio_engine
            .set_spatial_listener(listener, current_camera.view_direction(), current_camera.look_up_vector());
        self.audio_engine.update();
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
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
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
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

        // The item service windows are answered with a request packet. Closing
        // one with its X button does not answer it, and the identify list in
        // particular leaves `WIP_DISABLE_ALL` on the server, which blocks item
        // and skill use until the request or the official "cancel" value
        // arrives. Send the matching cancel request once the window is gone.
        if !self.interface.is_window_with_class_open(WindowClass::ItemService) {
            let kind = *self.client_state.follow(client_state().item_service().kind());

            if let Some(kind) = kind {
                match kind {
                    ItemServiceKind::ItemIdentify => {
                        let _ = self.networking_system.cancel_identify_item();
                    }
                    ItemServiceKind::WeaponRefine => {
                        let _ = self.networking_system.cancel_weapon_refine();
                    }
                    ItemServiceKind::ItemRepair => {
                        let _ = self.networking_system.cancel_repair_item();
                    }
                    ItemServiceKind::CardComposition => {}
                }

                self.client_state.follow_mut(client_state().item_service()).clear();
            }
        }

        // The NPC refine UI is opened by a server packet, but closing the window
        // with its X button does not tell the server. rAthena keeps
        // `state.refineui_open` set and would then refuse to open the UI again
        // (`clif.cpp:22485`/`22495`), so send the close packet once the window
        // is gone.
        if *self.client_state.follow(client_state().item_service().refining().open())
            && !self.interface.is_window_with_class_open(WindowClass::Refining)
        {
            let _ = self.networking_system.close_refining_ui();
            self.client_state.follow_mut(client_state().item_service().refining()).clear();
        }

        // Some debug windows, such as the packet history or cache statistics, require
        // special update logic.
        #[cfg(feature = "debug")]
        self.update_debug_windows(delta_time);

        // We run this last to give the loader thread as much time as possible to
        // complete the loading. When starting the actual render process, we
        // can't modify resources anymore until the next frame.
        self.update_loaded_resources(client_tick);

        // The character previews are requested after the load results were
        // applied, so an animation that already finished loading in this frame
        // is not requested again.
        self.update_character_selection_previews();
        self.update_character_selection_cooldown();

        // Headless benchmark driver (`RO_NEXT_BENCH`, see `benchmark.rs`); a
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

                if previous_position != Some(player_position) {
                    self.refresh_navigation_route();
                }
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
        self.update_status_auras();
        self.update_status_visuals();

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

        // Update the Asura Strike banner.
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

        // Resolve the PvP/GvG/PK target rule before `point_light_manager` is
        // borrowed mutably; the cursor decision below only needs this flag.
        let hovered_entity_attackable = match input_report.mouse_target {
            PickerTarget::Entity(entity_id) => self.can_attack_entity(entity_id),
            _ => false,
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
                        let entity_type = self
                            .client_state
                            .follow(client_state().entities())
                            .iter()
                            .find(|entity| entity.get_entity_id() == entity_id)
                            .map(|entity| entity.get_entity_type());

                        match entity_type {
                            Some(EntityType::Npc) => MouseCursorState::Dialog,
                            Some(EntityType::Warp) => MouseCursorState::Warp,
                            Some(EntityType::Monster) => MouseCursorState::Attack,
                            // The official client only shows the attack cursor
                            // on a player when the PvP/GvG/PK rule allows the
                            // attack; otherwise it keeps the default cursor
                            // (`CGameMode::HandleActorPick` `SetCursorAction(0)`).
                            Some(EntityType::Player) if hovered_entity_attackable => MouseCursorState::Attack,
                            _ => MouseCursorState::Default,
                        }
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
                        #[cfg_attr(feature = "debug", ro_next_debug::debug_condition(!render_options.use_debug_camera))]
                        interface_frame.set_mouse_mode(MouseInputMode::RotateCamera);
                    } else if mouse_button == MouseButton::DoubleRight && currently_playing {
                        #[cfg_attr(feature = "debug", ro_next_debug::debug_condition(!render_options.use_debug_camera))]
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
                        .is_some_and(|entity| {
                            matches!(
                                entity.get_entity_type(),
                                EntityType::Player | EntityType::Pet | EntityType::Homunculus | EntityType::Mercenary
                            )
                        });
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
                    #[cfg_attr(feature = "debug", ro_next_debug::debug_condition(!render_options.use_debug_camera))]
                    self.input_event_buffer.push(InputEvent::ZoomCamera { zoom_factor: delta });
                }
            }

            // Focus the chat if the interface is not focused, no other element is capturing
            // the keyboard input, enter was pressed, and the chat
            // window is open. Enter arrives as the `\x0d` text character on most
            // platforms; the data-driven chat command key binding
            // (`data\client\keybinds.ron`) covers the platforms/backends where the
            // physical key does not produce a text character.
            if (!interface_has_focus || !interface_frame.input_characters(&self.client_state, &input_report.characters))
                && (input_report.characters.contains(&'\x0d') || self.input_system.chat_command_key_pressed())
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

        // Drop warp portals whose server removal packet was lost (safety net; the
        // normal removal is the `RemoveSkillUnit` packet).
        {
            let now = std::time::Instant::now();
            self.warp_skill_units.retain(|(_, _, _, expires_at)| *expires_at > now);
        }

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
                head_marks: &self.head_marks,
                cast_bars: &self.cast_bars,
                skill_name_texts: &self.skill_name_texts,
                warp_portal: self.warp_portal.as_ref(),
                weather: self.weather.as_ref(),
                spirit_sphere_texture: self.spirit_sphere_texture.as_ref(),
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

        // Sky/background and scene fog from `mapskydata.lub`. Maps without sky
        // data keep the black background, so existing maps render exactly as
        // before. `BG_Fog` fades the geometry into the same colour (it is not
        // enabled by this data set, the guarded path stays nevertheless).
        let background_color = map.sky_background_color().unwrap_or(Color::BLACK);
        let scene_fog = map.scene_fog();

        let uniforms = Uniforms {
            view_matrix,
            projection_matrix,
            camera_position,
            animation_timer_ms,
            ambient_light_color: map.ambient_light_color(),
            fog_color: scene_fog.map(|fog| Color { alpha: 1.0, ..fog.color }).unwrap_or(Color::TRANSPARENT),
            fog_range: scene_fog.map(|fog| [fog.near, fog.far]).unwrap_or([0.0, 0.0]),
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
            background_color,
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

/// Returns `true` if the given environment variable is set to a truthy value.
fn environment_flag(variable: &str) -> bool {
    std::env::var(variable).is_ok_and(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
}

/// The deadline of the first log line for `RO_NEXT_FPS_LOG`, if enabled.
fn frames_per_second_log_deadline() -> Option<Instant> {
    if environment_flag(FRAMERATE_LOG_ENVIRONMENT_VARIABLE) {
        Some(Instant::now() + Duration::from_secs(FRAMERATE_LOG_INTERVAL_SECONDS))
    } else {
        None
    }
}

/// Environment variable that enables the periodic layout/text cache profiling
/// line (`[perf] ...`).
const PERFORMANCE_LOG_ENVIRONMENT_VARIABLE: &str = "RO_NEXT_PERF_LOG";
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

/// Frame phase profiling counters for `RO_NEXT_PERF_LOG`.
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
                            "[display] hint: use RO_NEXT_X11=1 to force X11/XWayland or RO_NEXT_WAYLAND=1 to force native Wayland"
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
        // The `RO_NEXT_UI_SCALE` environment variable always has precedence.
        if self.initialize_interface_scaling_from_dpi {
            self.initialize_interface_scaling_from_dpi = false;

            if std::env::var("RO_NEXT_UI_SCALE").is_err()
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

/// Whether the given job may move while casting because of the Sage/Scholar/
/// Professor `Free Cast` passive.
///
/// Both sides are data: the learned skill has to carry the
/// `move_while_casting` behaviour flag (`skills.ron`, Free Cast 278) and the
/// job has to carry the `move_while_casting` capability (`jobs.ron`, Sage 16 /
/// Professor 4017). A passive that a job cannot learn therefore never applies,
/// and an unknown job or skill list defaults to "rooted" (the safe behaviour).
fn can_move_while_casting(job_id: JobId, skills: &[LearnedSkill]) -> bool {
    crate::world::jobs::can_move_while_casting(job_id)
        && skills
            .iter()
            .any(|skill| crate::world::skills::grants_move_while_casting(skill.skill_id))
}

impl Client {
    /// Plays the official "use item" animation (effect `EF_EXIT`): a short
    /// translucent column (`effect\alpha_down.tga`), rising sparkles
    /// (`이팩트\particle1`) and the `_heal_effect.wav` sound at the user.
    ///
    /// The effect is spawned when a consumable is used successfully
    /// (`ZC_USE_ITEM_ACK` with a non-zero result). Item ids above 600 use the
    /// effect, matching the official client's `IsAboveItemId600` check.
    fn spawn_item_use_effect(&mut self, entity_id: EntityId) {
        let Some(position) = self
            .client_state
            .follow(client_state().entities())
            .iter()
            .find(|entity| entity.get_entity_id() == entity_id)
            .map(|entity| entity.get_position())
        else {
            return;
        };

        let center = EffectCenter::Entity(entity_id, position);

        // Translucent column: 35 units high, 4.5 unit radii, ~0.35 alpha,
        // growing from the ground (`CRagEffect::Exit`).
        let params = PrimitiveParams {
            texture: "effect\\alpha_down.tga",
            seed: 0,
            count: 1,
            duration_ms: 600,
            delay_ms: 0,
            size: 9.0,
            size_end: 9.0,
            spread: 0.0,
            height: 35.0,
            bottom_radius: 4.5,
            top_radius: 4.5,
            alpha: 0.35,
            color: Color::WHITE,
            additive: false,
            rotation_speed: 0.0,
            grow: AuraGrow::Height,
        };

        if let Some(aura) = spawn_ground_aura(&self.texture_loader, center.clone(), &params) {
            self.effect_holder.add_effect(Box::new(aura));
        }

        // Rising sparkle sprite (`particle1.spr/.act`): the official effect
        // spawns a particle every ninth tick for the duration of the column.
        let sprite_params = SpriteEffectParams {
            files: &["이팩트\\particle1"],
            offset: Vector3::new(0.0, 6.0, 0.0),
            scale: 1.0,
            duration_ms: 600,
            color: Color::WHITE,
            additive: true,
        };

        if let Some(sprite) = SpriteEffect::spawn(&self.sprite_loader, &self.action_loader, center, &sprite_params) {
            self.effect_holder.add_effect(Box::new(sprite));
        }

        let sound_effect = self.audio_engine.load("data\\wav\\_heal_effect.wav");
        self.audio_engine.play_spatial_sound_effect(sound_effect, position, 250.0);
    }
}

/// Requests the equipment animation layers (weapon, shield, robe) for
/// `entity` through the data-driven appearance modules and applies the layers
/// that are already available.
///
/// Every packet path that changes the body/equipment appearance (character
/// login, entity spawn, job change, equipment change) goes through this
/// function. The candidate paths come from [`crate::world::weapon`],
/// [`crate::world::shield`] and [`crate::world::robe`], so no call site knows
/// weapon types, item view ids or sprite suffixes.
fn request_equipment_layers(async_loader: &AsyncLoader, entity: &mut Entity) {
    request_weapon_animation(async_loader, entity);
    request_shield_animation(async_loader, entity);
    request_robe_animation(async_loader, entity);
}

/// Requests the equipped weapon animation layer for `entity`.
fn request_weapon_animation(async_loader: &AsyncLoader, entity: &mut Entity) {
    let profile = crate::world::weapon::profile(entity.get_weapon());
    let candidates = entity.get_weapon_part_candidates();

    crate::logging::log_debug!(
        "weapon layer: entity {} kind {:?} ({}) range {} ammo {:?} -> {} candidate(s)",
        entity.get_entity_id().0,
        profile.kind(),
        profile.subtype(),
        profile.attack_range(),
        profile.ammo(),
        candidates.len()
    );

    // A player without any candidate path would silently lose the weapon
    // layer; report it once per entity so a missing/custom weapon sprite is
    // visible in the normal log instead of a debug-only line.
    if candidates.is_empty() && entity.get_entity_type() == crate::world::EntityType::Player {
        crate::logging::log_warn!(
            "weapon layer: entity {} has no sprite candidates (weapon view id {}, kind {:?}, subtype {:?})",
            entity.get_entity_id().0,
            entity.get_weapon(),
            profile.kind(),
            profile.subtype()
        );
    }

    if candidates.is_empty() || entity.has_weapon_animation() {
        return;
    }

    if let Some(weapon_animation) = async_loader.request_weapon_animation_data_load(entity.get_entity_id(), candidates) {
        entity.set_weapon_animation(weapon_animation);
    }
}

/// Requests the equipped shield animation layer for `entity`.
fn request_shield_animation(async_loader: &AsyncLoader, entity: &mut Entity) {
    let candidates = entity.get_shield_part_candidates();

    if candidates.is_empty() || entity.has_shield_animation() {
        return;
    }

    if let Some(shield_animation) = async_loader.request_shield_animation_data_load(entity.get_entity_id(), candidates) {
        entity.set_shield_animation(shield_animation);
    }
}

/// Requests the equipped robe animation layer for `entity`.
fn request_robe_animation(async_loader: &AsyncLoader, entity: &mut Entity) {
    let candidates = entity.get_robe_part_candidates();

    if candidates.is_empty() || entity.has_robe_animation() {
        return;
    }

    if let Some(robe_animation) = async_loader.request_robe_animation_data_load(entity.get_entity_id(), candidates) {
        entity.set_robe_animation(robe_animation);
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_bytes::{ByteWriter, ToBytes};
    use ragnarok_packets::{AmmunitionActionType, AttackRange, BuyItemResult, JobId, SkillId, SkillLevel, SkillType};

    use super::{ammunition_action_code, buy_item_result_code, can_move_while_casting, cutin_texture_candidates, is_same_map_teleport};
    use crate::interface::windows::book_pages;
    use crate::state::lifecycle::SessionState;
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
    fn free_cast_requires_the_data_driven_skill_flag_and_job_capability() {
        // Sage with the passive learned (skills.ron `move_while_casting` flag
        // + jobs.ron `move_while_casting` capability).
        assert!(can_move_while_casting(JobId(16), &[learned(278)]));
        // Professor (rebirth Sage) with the passive learned.
        assert!(can_move_while_casting(JobId(4017), &[learned(278)]));

        // Non-Sage jobs are never allowed, even with the skill in the list.
        assert!(!can_move_while_casting(JobId(8), &[learned(278)]));
        assert!(!can_move_while_casting(JobId(0), &[learned(278)]));

        // Sage without the passive is rooted.
        assert!(!can_move_while_casting(JobId(16), &[]));
        assert!(!can_move_while_casting(JobId(16), &[learned(28), learned(34)]));
    }

    #[test]
    fn unknown_jobs_default_to_rooted() {
        assert!(!can_move_while_casting(JobId(9999), &[learned(278)]));
        // The behaviour is a data lookup, not a skill-id branch: an unknown
        // skill never grants it.
        assert!(!can_move_while_casting(JobId(16), &[learned(65_500)]));
    }

    #[test]
    fn job_registry_data_is_valid() {
        let rows = super::check_jobs(None).expect("the committed job data has to be valid");
        assert_eq!(rows, 181, "one row per real rAthena e_job id");
        assert!(crate::world::jobs::profile(JobId(4252)).is_some());
        assert!(crate::world::jobs::profile(JobId(65535)).is_none());
    }

    #[test]
    fn job_registry_check_rejects_malformed_data() {
        let directory = std::env::temp_dir();
        let path = directory.join(format!("ro-next-jobs-check-{}.ron", std::process::id()));
        std::fs::write(&path, "(jobs: [)").expect("failed to write the malformed fixture");

        let result = super::check_jobs(Some(&path));
        let _ = std::fs::remove_file(&path);

        let error = result.expect_err("malformed data must be rejected");
        assert!(error.contains("schema error"), "{error}");
    }

    #[test]
    fn job_registry_check_reports_a_missing_file() {
        let path = std::path::Path::new("/nonexistent/ro-next/jobs.ron");
        let error = super::check_jobs(Some(path)).expect_err("a missing file must be rejected");
        assert!(error.contains("failed to read"), "{error}");
    }

    #[test]
    fn alias_data_is_valid() {
        let summary = super::check_aliases(None).expect("the committed alias data has to be valid");
        assert_eq!(
            summary.job_skill_tree_fallbacks, 56,
            "one fallback per server job with an empty client tree"
        );
        assert_eq!(
            summary.monster_sprite_aliases, 232,
            "one alias per monster without an exact sprite"
        );
        assert_eq!(summary.skill_icon_aliases, 241, "one alias per skill without an exact icon");
    }

    #[test]
    fn alias_check_rejects_malformed_data() {
        let directory = std::env::temp_dir();
        let path = directory.join(format!("ro-next-aliases-check-{}.ron", std::process::id()));
        std::fs::write(&path, "(version: 1,").expect("failed to write the malformed fixture");

        let result = super::check_aliases(Some(&path));
        let _ = std::fs::remove_file(&path);

        let error = result.expect_err("malformed data must be rejected");
        assert!(error.contains("schema error"), "{error}");
    }

    #[test]
    fn keybind_data_is_valid() {
        let bindings = super::check_keybinds(None).expect("the committed keybind data has to be valid");
        // 19 shortcut entries + 9 emotion keys + 9 hotbar slots + 2 chat
        // command keys.
        assert_eq!(bindings, 39);
        assert_eq!(crate::input::keybinds::shortcuts().len(), 19);
        assert_eq!(crate::input::keybinds::hotbar_keys().len(), 9);
    }

    #[test]
    fn keybind_check_rejects_malformed_data() {
        let directory = std::env::temp_dir();
        let path = directory.join(format!("ro-next-keybinds-check-{}.ron", std::process::id()));
        std::fs::write(&path, "(shortcuts: [)").expect("failed to write the malformed fixture");

        let result = super::check_keybinds(Some(&path));
        let _ = std::fs::remove_file(&path);

        let error = result.expect_err("malformed data must be rejected");
        assert!(error.contains("schema error"), "{error}");
    }

    fn alias_check_reports_a_missing_file() {
        let path = std::path::Path::new("/nonexistent/ro-next/aliases.ron");
        let error = super::check_aliases(Some(path)).expect_err("a missing file must be rejected");
        assert!(error.contains("failed to read"), "{error}");
    }

    #[test]
    fn keybind_check_reports_a_missing_file() {
        let path = std::path::Path::new("/nonexistent/ro-next/keybinds.ron");
        let error = super::check_keybinds(Some(path)).expect_err("a missing file must be rejected");
        assert!(error.contains("failed to read"), "{error}");
    }

    #[test]
    fn paper_doll_data_is_valid() {
        let slots = super::check_paper_doll(None).expect("the committed paper-doll data has to be valid");
        assert_eq!(slots, 21, "one slot per equipment position (11 classic + 10 costume/shadow)");
    }

    #[test]
    fn paper_doll_check_rejects_malformed_data() {
        let directory = std::env::temp_dir();
        let path = directory.join(format!("ro-next-paper-doll-check-{}.ron", std::process::id()));
        std::fs::write(&path, "(slots: [)").expect("failed to write the malformed fixture");

        let result = super::check_paper_doll(Some(&path));
        let _ = std::fs::remove_file(&path);

        let error = result.expect_err("malformed data must be rejected");
        assert!(error.contains("schema error"), "{error}");
    }

    #[test]
    fn paper_doll_check_reports_a_missing_file() {
        let path = std::path::Path::new("/nonexistent/ro-next/paper_doll_slots.ron");
        let error = super::check_paper_doll(Some(path)).expect_err("a missing file must be rejected");
        assert!(error.contains("failed to read"), "{error}");
    }

    #[test]
    fn skill_behaviour_data_is_valid() {
        let summary = super::check_skills(None).expect("the committed skill data has to be valid");
        assert_eq!(summary.behaviours, 3, "Rolling Cutter, Warp Portal and Free Cast");
        assert_eq!(summary.clouds, 2, "the cloud row covers both Storm Gust ids");
        assert!(crate::world::skills::channels(SkillId(2036)));
        assert!(crate::world::skills::has_warp_target(SkillId(27)));
        assert!(crate::world::skills::grants_move_while_casting(SkillId(278)));
    }

    #[test]
    fn skill_behaviour_check_rejects_malformed_data() {
        let directory = std::env::temp_dir();
        let path = directory.join(format!("ro-next-skills-check-{}.ron", std::process::id()));
        std::fs::write(&path, "(behaviours: [)").expect("failed to write the malformed fixture");

        let result = super::check_skills(Some(&path));
        let _ = std::fs::remove_file(&path);

        let error = result.expect_err("malformed data must be rejected");
        assert!(error.contains("schema error"), "{error}");
    }

    #[test]
    fn skill_behaviour_check_reports_a_missing_file() {
        let path = std::path::Path::new("/nonexistent/ro-next/skills.ron");
        let error = super::check_skills(Some(path)).expect_err("a missing file must be rejected");
        assert!(error.contains("failed to read"), "{error}");
    }

    /// Regression test for the logout -> re-select black screen: after logging
    /// out the client keeps the name of the last map but has no loaded map and
    /// is in `MapLoading`. A `ChangeMap` with the same name is a *fresh map
    /// entry* in that situation and must take the full map load path instead of
    /// the same-map teleport shortcut (which skipped the load and left the
    /// client on a black screen).
    #[test]
    fn fresh_character_map_entry_is_not_a_same_map_teleport() {
        assert!(!is_same_map_teleport(
            Some("prontera"),
            "prontera",
            false,
            SessionState::MapLoading
        ));
        // Even if a menu background map finished loading, the session is still
        // in the map entry and the character map has to be loaded.
        assert!(!is_same_map_teleport(
            Some("prontera"),
            "prontera",
            true,
            SessionState::MapLoading
        ));
        assert!(!is_same_map_teleport(None, "prontera", false, SessionState::MapLoading));
    }

    #[test]
    fn in_game_same_map_move_is_a_teleport() {
        assert!(is_same_map_teleport(Some("prontera"), "prontera", true, SessionState::InGame));
        // Different map, missing map data or not in the game: never a
        // same-map teleport.
        assert!(!is_same_map_teleport(Some("prontera"), "geffen", true, SessionState::InGame));
        assert!(!is_same_map_teleport(Some("prontera"), "prontera", false, SessionState::InGame));
        assert!(!is_same_map_teleport(
            Some("prontera"),
            "prontera",
            true,
            SessionState::CharSelect
        ));
    }

    /// The little-endian numeric value an enum is serialized to; the message
    /// table keys are exactly this wire value.
    fn wire_code<T: ToBytes>(value: &T) -> u16 {
        let mut writer = ByteWriter::new();
        value.to_bytes(&mut writer).expect("test-value encoding cannot fail");
        let bytes = writer.into_inner();
        match bytes.len() {
            1 => u16::from(bytes[0]),
            2 => u16::from_le_bytes([bytes[0], bytes[1]]),
            4 => u16::try_from(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])).expect("test code fits into u16"),
            length => panic!("unexpected enum width: {length}"),
        }
    }

    #[test]
    fn buy_item_result_codes_match_the_wire_values() {
        for (result, code) in [
            (BuyItemResult::NotEoughZeny, 1),
            (BuyItemResult::WeightLimitExceeded, 2),
            (BuyItemResult::TooManyItems, 3),
            (BuyItemResult::TooManyOfThisItem, 9),
            (BuyItemResult::PropsOpenAir, 10),
            (BuyItemResult::ExchangeFailed, 11),
            (BuyItemResult::ExchangeWellDone, 12),
            (BuyItemResult::ItemSoldOut, 13),
            (BuyItemResult::NotEnoughGoods, 14),
        ] {
            assert_eq!(buy_item_result_code(&result), code);
            assert_eq!(wire_code(&result), code, "wire drift for {result:?}");
        }
    }

    #[test]
    fn ammunition_action_codes_match_the_wire_values() {
        for (action, code) in [
            (AmmunitionActionType::EquipProperAmmunitionFirst, 0),
            (AmmunitionActionType::WeightLimitExceeded1, 1),
            (AmmunitionActionType::WeightLimitExceeded2, 2),
            (AmmunitionActionType::Equipped, 3),
        ] {
            assert_eq!(ammunition_action_code(&action), code);
            assert_eq!(wire_code(&action), code, "wire drift for {action:?}");
        }
    }
    #[test]
    fn cutin_names_resolve_to_the_classic_illust_directory_first() {
        let candidates = cutin_texture_candidates("kafra_01");

        assert_eq!(candidates[0], "유저인터페이스\\illust\\kafra_01.bmp");
        assert!(!candidates.is_empty());
        // Monster/NPC portrait fallbacks are present for servers that send a
        // sprite name instead of a cut-in name.
        assert!(candidates.iter().any(|candidate| candidate == "monster\\kafra_01.bmp"));
        assert!(candidates.iter().any(|candidate| candidate == "npc\\kafra_01.bmp"));
    }

    #[test]
    fn cutin_paths_with_extension_are_kept_verbatim() {
        let candidates = cutin_texture_candidates("monster\\1002.bmp");
        assert_eq!(candidates[0], "monster\\1002.bmp");
        assert!(cutin_texture_candidates("   ").is_empty());
    }

    #[test]
    fn book_pages_drop_the_background_command_and_split_by_line_count() {
        let mut text = String::from("%faf0e6\r\n");
        for index in 0..32 {
            text.push_str(&format!("line {index}\r\n"));
        }

        let pages = book_pages(&text);
        assert_eq!(pages.len(), 3);
        assert_eq!(pages[0].len(), crate::interface::windows::book::BOOK_LINES_PER_PAGE);
        assert_eq!(pages[0][0], "line 0");
        assert_eq!(pages[2].len(), 2);
        assert_eq!(pages[2][1], "line 31");
        assert!(book_pages("").is_empty());
    }
}
